//! Optional native schema-asset delivery fixture. Default release builds retain
//! their embedded catalog; this tool emits immutable original source bytes.
use cem_ml::schema::asset_pack::{builtin_schema_asset_pack_export, schema_asset_sha256};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityFile {
    path: String,
    bytes: usize,
    sha256: String,
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [flag, path] if flag == "--registry-out" || flag == "--check-registry" => {
            registry(Path::new(path), flag == "--check-registry")
        }
        _ => output_directory().and_then(|output| emit(&output)),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cem-ml-schema-assets-emit: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Emit/check named native identity metadata using the complete source build.
fn registry(path: &Path, check: bool) -> Result<(), String> {
    if cfg!(feature = "external-schema-assets") {
        return Err("registry emission requires the complete embedded source catalog".into());
    }
    let export = builtin_schema_asset_pack_export().map_err(message)?;
    let bytes = json_bytes(&export.index.packages)?;
    if check {
        if std::fs::read(path).map_err(message)? != bytes {
            return Err("compiled registry differs from complete native sources; regenerate with --registry-out".into());
        }
        Ok(())
    } else { std::fs::write(path, bytes).map_err(message) }
}

fn emit(output: &Path) -> Result<(), String> {
    if output.exists() && std::fs::read_dir(output).map_err(message)?.next().is_some() {
        return Err("output must be absent or empty".into());
    }
    let export = builtin_schema_asset_pack_export().map_err(message)?;
    let index = export.index.to_bytes().map_err(message)?;
    let index_digest = schema_asset_sha256(&index);
    let mut files = BTreeMap::<String, Vec<u8>>::new();
    let mut kinds = BTreeMap::<String, (usize, usize)>::new();
    let mut source_bytes = 0usize;
    for source in &export.sources {
        let bytes = source.source.as_bytes();
        source_bytes += bytes.len();
        let kind = serde_json::to_value(&source.descriptor.kind).map_err(message)?;
        let totals = kinds
            .entry(kind.as_str().ok_or("invalid source kind")?.into())
            .or_default();
        totals.0 += 1;
        totals.1 += bytes.len();
        let previous = files.insert(source.descriptor.asset_path(), bytes.to_vec());
        if previous
            .as_deref()
            .is_some_and(|previous| previous != bytes)
        {
            return Err("digest-addressed source collision".into());
        }
    }
    let unique_file_count = files.len();
    let unique_file_bytes: usize = files.values().map(Vec::len).sum();
    let measurement = serde_json::json!({
        "formatVersion": 1,
        "commonVersion": cem_ml::VERSION,
        "packageCount": export.index.packages.len(),
        "sourceCount": export.sources.len(),
        "sourceBytes": source_bytes,
        "digestFileCount": unique_file_count,
        "digestFileBytes": unique_file_bytes,
        "indexBytes": index.len(),
        "indexSha256": index_digest,
        "sourceKinds": kinds.into_iter().map(|(kind, (count, bytes))| (kind, serde_json::json!({"count":count,"bytes":bytes}))).collect::<BTreeMap<_,_>>(),
        "embeddedFallbackRetained": true,
        "binaryPartitioned": false,
        "checkpointSavingsBytes": 0,
    });
    let bootstrap = serde_json::json!({
        "indexDigest": index_digest,
        "indexBytes": index.len(),
        "packageCount": export.index.packages.len(),
        "sourceCount": export.sources.len(),
        "sourceBytes": source_bytes,
    });
    files.insert("bootstrap.json".into(), json_bytes(&bootstrap)?);
    files.insert("index.json".into(), index);
    files.insert(
        "index.sha256".into(),
        format!("{index_digest}\n").into_bytes(),
    );
    files.insert("measurements.json".into(), json_bytes(&measurement)?);
    let integrity_files: Vec<_> = files
        .iter()
        .map(|(path, bytes)| IntegrityFile {
            path: path.clone(),
            bytes: bytes.len(),
            sha256: schema_asset_sha256(bytes),
        })
        .collect();
    let integrity = serde_json::json!({
        "formatVersion": 1,
        "commonVersion": cem_ml::VERSION,
        "algorithm": "sha256",
        "indexSha256": index_digest,
        "files": integrity_files,
        "offlineUrls": files.keys().map(String::as_str).chain(std::iter::once("integrity.json")).collect::<Vec<_>>(),
    });
    files.insert("integrity.json".into(), json_bytes(&integrity)?);
    for (path, bytes) in &files {
        let target = output.join(path);
        std::fs::create_dir_all(target.parent().ok_or("missing output parent")?)
            .map_err(message)?;
        std::fs::write(target, bytes).map_err(message)?;
    }
    println!(
        "Emitted {} native schema sources in {} immutable files to {}",
        export.sources.len(),
        files.len(),
        output.display()
    );
    Ok(())
}

fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(message)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn message(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn output_directory() -> Result<PathBuf, String> {
    let mut arguments = std::env::args_os().skip(1);
    match (
        arguments.next().as_deref(),
        arguments.next(),
        arguments.next(),
    ) {
        (Some(flag), Some(path), None) if flag == "--out" => Ok(PathBuf::from(path)),
        _ => Err("usage: cem-ml-schema-assets-emit --out <directory>".into()),
    }
}
