//! Deterministic TypeScript projections for the command-service wire contract.
//!
//! The exported declarations are derived from the same serde-annotated Rust
//! types used by native and WASM execution. The small handwritten section in
//! `index.d.ts` describes only JavaScript callback and transfer envelopes that
//! do not have a serializable Rust representation.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ts_rs::{Config, TS};

use crate::capability::CapabilityRequest;
use crate::command_artifact::{CommandServiceArtifactDisposeAckV1, CommandServiceArtifactReadV1};
use crate::command_host::{CommandResolvedResourceV1, CommandResourceReadRequestV1};
use crate::command_publication::{
    CommandPreparedWriteTokenV1, CommandResolvedWriteV1, CommandResourceWriteRequestV1,
    CommandRevisionLedgerRequestV1,
};
use crate::command_runtime::{CommandServiceControlAckV1, CommandServiceProgressV1};
use crate::command_service::{
    CommandRevisionLedgerV1, CommandServiceRequestV1, CommandServiceResultV1,
};

pub const COMMAND_TYPES_INDEX: &str = "index.d.ts";
static NEXT_TEMPORARY_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Replace `output_directory` with a complete, byte-stable declaration tree.
pub fn emit_command_service_types_v1(output_directory: &Path) -> Result<(), String> {
    validate_output_directory(output_directory)?;
    let temporary = temporary_output_directory(output_directory)?;
    if temporary.exists() {
        fs::remove_dir_all(&temporary)
            .map_err(|error| io_error("clear temporary output", error))?;
    }
    fs::create_dir_all(&temporary).map_err(|error| io_error("create temporary output", error))?;

    let result = emit_into(&temporary);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&temporary);
        return Err(error);
    }

    if output_directory.exists() {
        fs::remove_dir_all(output_directory)
            .map_err(|error| io_error("replace declaration output", error))?;
    }
    fs::rename(&temporary, output_directory)
        .map_err(|error| io_error("install declaration output", error))?;
    Ok(())
}

fn emit_into(output_directory: &Path) -> Result<(), String> {
    let config = Config::new()
        .with_out_dir(output_directory)
        .with_large_int("number")
        .with_import_extension(Some("js"));

    export::<CapabilityRequest>(&config)?;
    export::<CommandServiceRequestV1>(&config)?;
    export::<CommandServiceResultV1>(&config)?;
    export::<CommandRevisionLedgerRequestV1>(&config)?;
    export::<CommandRevisionLedgerV1>(&config)?;
    export::<CommandResourceReadRequestV1>(&config)?;
    export::<CommandResolvedResourceV1>(&config)?;
    export::<CommandResourceWriteRequestV1>(&config)?;
    export::<CommandPreparedWriteTokenV1>(&config)?;
    export::<CommandResolvedWriteV1>(&config)?;
    export::<CommandServiceProgressV1>(&config)?;
    export::<CommandServiceControlAckV1>(&config)?;
    export::<CommandServiceArtifactReadV1>(&config)?;
    export::<CommandServiceArtifactDisposeAckV1>(&config)?;

    let generated = rename_declarations(output_directory)?;
    fs::write(
        output_directory.join(COMMAND_TYPES_INDEX),
        render_index(&generated),
    )
    .map_err(|error| io_error("write declaration index", error))?;
    Ok(())
}

fn export<T: TS + 'static>(config: &Config) -> Result<(), String> {
    T::export_all(config).map_err(|error| format!("generate {}: {error}", T::ident(config)))
}

fn rename_declarations(output_directory: &Path) -> Result<Vec<String>, String> {
    let mut source_paths = Vec::new();
    collect_files(output_directory, &mut source_paths)?;
    source_paths.sort();

    let mut modules = Vec::with_capacity(source_paths.len());
    for source_path in source_paths {
        if source_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("ts")
        {
            return Err(format!(
                "unexpected generated declaration path `{}`",
                source_path.display()
            ));
        }
        let name = source_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                format!(
                    "invalid generated declaration path `{}`",
                    source_path.display()
                )
            })?
            .to_owned();
        let declaration_path = source_path.with_file_name(format!("{name}.d.ts"));
        fs::rename(&source_path, declaration_path)
            .map_err(|error| io_error("rename generated declaration", error))?;
        let relative = source_path
            .strip_prefix(output_directory)
            .map_err(|error| format!("project generated declaration path: {error}"))?
            .with_extension("");
        modules.push(relative.to_string_lossy().replace('\\', "/"));
    }
    Ok(modules)
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(directory).map_err(|error| io_error("read declaration output", error))?;
    for entry in entries {
        let path = entry
            .map_err(|error| io_error("read generated declaration", error))?
            .path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn render_index(generated: &[String]) -> String {
    let mut output = String::from(
        "// Generated by cem-ml-command-types-emit from Rust serde wire declarations.\n\
         // Do not edit generated output.\n\n",
    );
    for module in generated {
        output.push_str(&format!("export type * from \"./{module}.js\";\n"));
    }
    output.push_str(
        r#"
import type { CommandPreparedWriteTokenV1 } from "./CommandPreparedWriteTokenV1.js";
import type { CommandResolvedResourceV1 } from "./CommandResolvedResourceV1.js";
import type { CommandResolvedWriteV1 } from "./CommandResolvedWriteV1.js";
import type { CommandResourceReadRequestV1 } from "./CommandResourceReadRequestV1.js";
import type { CommandResourceWriteRequestV1 } from "./CommandResourceWriteRequestV1.js";
import type { CommandRevisionLedgerRequestV1 } from "./CommandRevisionLedgerRequestV1.js";
import type { CommandRevisionLedgerV1 } from "./CommandRevisionLedgerV1.js";
import type { CommandServiceProgressV1 } from "./CommandServiceProgressV1.js";

export type CommandServiceMaybePromiseV1<T> = T | PromiseLike<T>;

export interface CommandServiceHostCapabilitiesV1 {
    readonly currentRevision: (
        request: CommandRevisionLedgerRequestV1,
    ) => CommandServiceMaybePromiseV1<CommandRevisionLedgerV1>;
    readonly readResource: (
        request: CommandResourceReadRequestV1,
    ) => CommandServiceMaybePromiseV1<CommandResolvedResourceV1>;
    readonly prepareWrite: (
        request: CommandResourceWriteRequestV1,
        bytes: Uint8Array,
    ) => CommandServiceMaybePromiseV1<CommandPreparedWriteTokenV1>;
    readonly commitWrite: (
        token: string,
    ) => CommandServiceMaybePromiseV1<CommandResolvedWriteV1>;
    readonly rollbackWrite: (token: string) => CommandServiceMaybePromiseV1<void>;
}

export type CommandServiceProgressCallbackV1 = (progress: CommandServiceProgressV1) => void;
export type CommandRevisionLedgerJsonCallbackV1 = (
    requestJson: string,
) => CommandServiceMaybePromiseV1<string>;
export type CommandResourceReadJsonCallbackV1 = (
    requestJson: string,
) => CommandServiceMaybePromiseV1<string>;
export type CommandPrepareWriteJsonCallbackV1 = (
    requestJson: string,
    bytes: Uint8Array,
) => CommandServiceMaybePromiseV1<string>;
export type CommandCommitWriteJsonCallbackV1 = (
    token: string,
) => CommandServiceMaybePromiseV1<string>;
export type CommandRollbackWriteJsonCallbackV1 = (
    token: string,
) => CommandServiceMaybePromiseV1<string | null | undefined>;
export type CommandProgressJsonCallbackV1 = (progressJson: string) => unknown;

export interface CommandArtifactReadWireResponseV1 {
    readonly json: string;
    readonly bytes?: Uint8Array;
}
"#,
    );
    output
}

fn validate_output_directory(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || path.parent().is_none() || path.file_name().is_none() {
        return Err("declaration output must name a scoped directory".to_owned());
    }
    Ok(())
}

fn temporary_output_directory(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "declaration output directory must be valid UTF-8".to_owned())?;
    let generation = NEXT_TEMPORARY_GENERATION.fetch_add(1, Ordering::Relaxed);
    Ok(path.with_file_name(format!(".{name}.tmp-{}-{generation}", std::process::id())))
}

fn io_error(action: &str, error: std::io::Error) -> String {
    format!("{action}: {error}")
}
