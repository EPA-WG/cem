#![cfg(feature = "typescript-projections")]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cem_ml::typescript::emit_command_service_types_v1;

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("fixture clock follows the Unix epoch")
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "cem-ml-command-types-{}-{nonce}",
            std::process::id()
        )))
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn generated_command_types_follow_the_serde_wire_contract() {
    let fixture = FixtureDirectory::new();
    emit_command_service_types_v1(&fixture.0).expect("command declarations generate");

    let files = declaration_files(&fixture.0);
    let request = text(&files, "CommandServiceRequestV1.d.ts");
    assert!(request.contains("protocolVersion: number"));
    assert!(request.contains("requestId: string"));
    assert!(request.contains("runPlan: CommandRunPlanV1"));
    assert!(request.contains("resourceVersions: CommandUriMapV1<CommandResourceVersionV1>"));

    let operations = text(&files, "PortableOperationRequestV1.d.ts");
    for tag in [
        "parse",
        "validate",
        "check",
        "inspect",
        "convert",
        "query",
        "transform",
        "trace",
        "version-capabilities",
    ] {
        assert!(operations.contains(&format!("\"kind\": \"{tag}\"")));
    }
    assert!(operations.contains("preserveSourceOffsets: boolean"));
    assert!(operations.contains("templateEntrypoint: TransformTemplateEntrypoint"));

    let result = text(&files, "CommandServiceResultV1.d.ts");
    assert!(result.contains("status: CommandServiceStatusV1"));
    assert!(result.contains("result?: CommandPayloadV1<PortableOperationResultV1> | null"));
    assert!(result.contains("artifacts: BoundedList<CommandArtifactHandleV1>"));
    assert!(result.contains("sourceMaps: BoundedList<CommandSourceMapReferenceV1>"));

    let progress = text(&files, "CommandServiceProgressV1.d.ts");
    assert!(progress.contains("operationId: OperationId"));
    assert!(progress.contains("sequence: number"));
    assert!(progress.contains("stage: CommandServiceProgressStageV1"));
    let control = text(&files, "CommandServiceControlAckV1.d.ts");
    assert!(control.contains("selectedScope: ExecutionScopeId"));
    assert!(control.contains("disposition: ControlAckDisposition"));
    let artifact = text(&files, "CommandServiceArtifactReadV1.d.ts");
    assert!(artifact.contains("handle: CommandArtifactHandleV1"));
    assert!(artifact.contains("byteLength: number"));
    assert!(artifact.contains("eof: boolean"));

    let parsed = text(&files, "ParsedCommandInvocationV1.d.ts");
    assert!(parsed.contains("schemaVersion: number"));
    assert!(parsed.contains("commandPath: Array<string>"));
    assert!(parsed.contains("globalOptions: { [key in string]: ParsedCommandValueV1 }"));
    let invocation = text(&files, "CommandInvocationBuildResponseV1.d.ts");
    assert!(invocation.contains("\"state\": \"needs-resources\""));
    assert!(invocation.contains("requirements: Array<CommandInvocationResourceRequirementV1>"));
    assert!(invocation.contains("\"state\": \"ready\""));
    let presentation = text(&files, "CommandPresentationWriteV1.d.ts");
    assert!(presentation.contains("target: CommandPresentationTargetKindV1"));
    assert!(presentation.contains("bytes: Array<number>"));

    let index = text(&files, "index.d.ts");
    assert!(index.contains("interface CommandServiceHostCapabilitiesV1"));
    assert!(index.contains("type CommandServiceProgressCallbackV1"));
    assert!(index.contains("type CommandPrepareWriteJsonCallbackV1"));
    assert!(index.contains("interface CommandArtifactReadWireResponseV1"));

    let combined = files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(!combined.contains(": any"));
    assert!(!combined.contains("Function"));
    assert!(!combined.contains("bigint"));
}

#[test]
fn generated_command_types_are_byte_stable() {
    let fixture = FixtureDirectory::new();
    emit_command_service_types_v1(&fixture.0).expect("first command declaration generation");
    let first = declaration_files(&fixture.0);
    emit_command_service_types_v1(&fixture.0).expect("second command declaration generation");
    let second = declaration_files(&fixture.0);
    assert_eq!(first, second);
}

fn declaration_files(root: &Path) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    collect_declarations(root, root, &mut files);
    files
}

fn collect_declarations(root: &Path, directory: &Path, files: &mut BTreeMap<String, String>) {
    let mut entries = fs::read_dir(directory)
        .expect("fixture declaration directory exists")
        .map(|entry| entry.expect("fixture declaration entry is readable").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_declarations(root, &path, files);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("fixture declaration stays under its root")
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(
                relative,
                fs::read_to_string(path).expect("fixture declaration is UTF-8"),
            );
        }
    }
}

fn text<'a>(files: &'a BTreeMap<String, String>, path: &str) -> &'a str {
    files
        .get(path)
        .unwrap_or_else(|| panic!("missing generated declaration `{path}`"))
}
