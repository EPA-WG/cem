//! Thin native transport for the shared pure project-bootstrap producer.
use crate::{
    cli,
    dispatch::{Outcome, Streams, EXIT_IO, EXIT_USAGE_OR_RESERVED},
};
use cem_ml::project_bootstrap::{project_bootstrap_json, PROJECT_BOOTSTRAP_MAX_REQUEST_BYTES};
use clap::Parser;
use std::io::Read;

pub fn run(args: cli::ProjectBootstrapArgs, streams: &mut Streams<'_>) -> Outcome {
    let read = || -> std::io::Result<Vec<u8>> {
        let reader: Box<dyn Read> = if args.request == std::path::Path::new("-") {
            Box::new(std::io::stdin())
        } else {
            Box::new(std::fs::File::open(&args.request)?)
        };
        let mut bytes = Vec::new();
        reader
            .take((PROJECT_BOOTSTRAP_MAX_REQUEST_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    let bytes = match read() {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(streams.stderr, "cem-ml: {error}");
            return Outcome::code(EXIT_IO);
        }
    };
    if streams.abort_signal.is_aborted() {
        return Outcome::code(130);
    }
    let request = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(error) => {
            let _ = writeln!(
                streams.stderr,
                "cem-ml: bootstrap request must be UTF-8: {error}"
            );
            return Outcome::code(EXIT_USAGE_OR_RESERVED);
        }
    };
    let mut response = project_bootstrap_json(request);
    // Clap also produces the browser grammar. Validate editable commands at
    // this boundary before returning a bundle, without a second grammar.
    let value: serde_json::Value = serde_json::from_str(&response).expect("native response JSON");
    if value.get("error").is_none() && value.get("bundle").is_some() {
        if let Err(message) = validate_commands(request) {
            response = serde_json::json!({"schemaVersion":value["schemaVersion"], "commonVersion":value["commonVersion"],
                "error":{"code":"cem.project_bootstrap.command_invalid","message":message}}).to_string();
        }
    }
    if streams.abort_signal.is_aborted() {
        return Outcome::code(130);
    }
    let failed = serde_json::from_str::<serde_json::Value>(&response)
        .map_or(true, |v| v.get("error").is_some());
    if writeln!(streams.stdout, "{response}").is_err() {
        return Outcome::code(EXIT_IO);
    }
    Outcome::code(if failed { EXIT_USAGE_OR_RESERVED } else { 0 })
}

fn validate_commands(request: &str) -> Result<(), String> {
    let request: serde_json::Value = serde_json::from_str(request).map_err(|e| e.to_string())?;
    let config: cem_ml::project_bootstrap::ProjectBootstrapConfiguration =
        serde_json::from_value(request["configuration"].clone()).map_err(|e| e.to_string())?;
    for resource in config
        .resources
        .into_iter()
        .filter(|r| r.role == cem_ml::studio_project::StudioProjectResourceRole::RunConfig)
    {
        let command: cem_ml::cli_command::CliCommandResource =
            serde_json::from_str(&resource.text).map_err(|e| e.to_string())?;
        if command.common_version != cem_ml::VERSION {
            return Err("Authored command must match the native common version".into());
        }
        let parsed =
            cli::Cli::try_parse_from(std::iter::once(command.binary_name).chain(command.argv))
                .map_err(|e| e.to_string())?;
        if !matches!(
            parsed.command,
            cli::Command::Parse(_)
                | cli::Command::Validate(_)
                | cli::Command::Inspect(_)
                | cli::Command::Convert(_)
                | cli::Command::Query(_)
                | cli::Command::Transform(_)
                | cli::Command::Trace(_)
        ) {
            return Err("Choose a portable project resource command".into());
        }
        if parsed.resolver_project.is_some() {
            return Err(
                "Authored commands use the project resolver supplied by the executing host".into(),
            );
        }
    }
    Ok(())
}
