//! Layer 2/3 semantic facts that are deterministic without import resolution.

use std::collections::BTreeMap;

use cem_ml::diagnostics::Diagnostic;
use cem_ml::source::ByteRange;
use serde_json::json;

use crate::diagnostics::{spanned_default, DECLARATION_DUPLICATE, IMPORT_ALIAS_DUPLICATE};
use crate::parser::{FunctionDecl, ImportDecl, QName, SurfaceModule, SurfaceNode, VariableDecl};

#[derive(Debug, Clone)]
struct SeenImportAlias {
    uri: String,
    range: ByteRange,
}

#[derive(Debug, Clone)]
struct SeenDeclaration {
    kind: &'static str,
    range: ByteRange,
}

pub fn validate_module_shape(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(validate_import_aliases(module));
    diagnostics.extend(validate_module_declarations(module));
    diagnostics
}

fn validate_import_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: BTreeMap<String, SeenImportAlias> = BTreeMap::new();
    for node in &module.nodes {
        let SurfaceNode::Import(import) = node else {
            continue;
        };
        let Some(alias) = import.alias.as_deref() else {
            continue;
        };
        if let Some(first) = seen.get(alias) {
            diagnostics.push(duplicate_import_alias_diagnostic(alias, import, first));
        } else {
            seen.insert(
                alias.to_owned(),
                SeenImportAlias {
                    uri: import.uri.clone(),
                    range: import.range,
                },
            );
        }
    }
    diagnostics
}

fn validate_module_declarations(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: BTreeMap<String, SeenDeclaration> = BTreeMap::new();
    for node in &module.nodes {
        let Some((name, kind, range)) = declaration_identity(node) else {
            continue;
        };
        if let Some(first) = seen.get(&name) {
            diagnostics.push(duplicate_declaration_diagnostic(&name, kind, range, first));
        } else {
            seen.insert(name, SeenDeclaration { kind, range });
        }
    }
    diagnostics
}

fn declaration_identity(node: &SurfaceNode) -> Option<(String, &'static str, ByteRange)> {
    match node {
        SurfaceNode::DeclareVariable(variable) => Some((
            qname_key(&variable.name),
            declaration_kind_for_variable(variable),
            variable.range,
        )),
        SurfaceNode::DeclareFunction(function) => Some((
            qname_key(&function.name),
            declaration_kind_for_function(function),
            function.range,
        )),
        _ => None,
    }
}

fn declaration_kind_for_variable(_variable: &VariableDecl) -> &'static str {
    "variable"
}

fn declaration_kind_for_function(_function: &FunctionDecl) -> &'static str {
    "function"
}

fn qname_key(name: &QName) -> String {
    match name.prefix.as_deref() {
        Some(prefix) => format!("{prefix}:{}", name.local),
        None => name.local.clone(),
    }
}

fn duplicate_import_alias_diagnostic(
    alias: &str,
    duplicate: &ImportDecl,
    first: &SeenImportAlias,
) -> Diagnostic {
    let mut diagnostic = spanned_default(
        IMPORT_ALIAS_DUPLICATE,
        format!(
            "duplicate CEM-QL import alias `{alias}`; first declared for `{}`",
            first.uri
        ),
        duplicate.range,
    );
    diagnostic.details = Some(json!({
        "behavior": "cem-ql-parse-report-fact",
        "factKind": "import-alias-duplicate",
        "contract": "unique-import-alias",
        "recoverable": true,
        "fatal": false,
        "alias": alias,
        "firstUri": first.uri,
        "firstRange": first.range,
        "duplicateUri": duplicate.uri,
        "duplicateRange": duplicate.range,
    }));
    diagnostic
}

fn duplicate_declaration_diagnostic(
    name: &str,
    duplicate_kind: &'static str,
    duplicate_range: ByteRange,
    first: &SeenDeclaration,
) -> Diagnostic {
    let mut diagnostic = spanned_default(
        DECLARATION_DUPLICATE,
        format!(
            "duplicate CEM-QL declaration `{name}`; first declared as {}",
            first.kind
        ),
        duplicate_range,
    );
    diagnostic.details = Some(json!({
        "behavior": "cem-ql-parse-report-fact",
        "factKind": "declaration-duplicate",
        "contract": "unique-declaration-name-per-scope",
        "recoverable": true,
        "fatal": false,
        "name": name,
        "firstKind": first.kind,
        "firstRange": first.range,
        "duplicateKind": duplicate_kind,
        "duplicateRange": duplicate_range,
    }));
    diagnostic
}

#[cfg(test)]
mod tests {
    use crate::api::parse;

    #[test]
    fn module_shape_reports_duplicate_import_aliases_after_first() {
        let parsed = parse(
            r#"module "https://example.test/queries/duplicates"

import "https://example.test/a" as ui
import "https://example.test/b" as ui
import "https://example.test/c" as other
"#,
        );

        let diagnostics = parsed
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "cem.ql.import_alias_duplicate")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 1, "{:?}", parsed.diagnostics);
        let diagnostic = diagnostics[0];
        assert_eq!(diagnostic.severity, cem_ml::diagnostics::Severity::Error);
        assert_eq!(
            diagnostic.details.as_ref().and_then(|details| {
                details.get("factKind").and_then(serde_json::Value::as_str)
            }),
            Some("import-alias-duplicate")
        );
        assert_eq!(
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("alias"))
                .and_then(serde_json::Value::as_str),
            Some("ui")
        );
    }

    #[test]
    fn module_shape_reports_duplicate_declarations_after_first() {
        let parsed = parse(
            r#"module "https://example.test/queries/duplicates"

declare let value = "first"
declare function value() { "second" }
declare function other() { value }
"#,
        );

        let diagnostics = parsed
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "cem.ql.declaration_duplicate")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 1, "{:?}", parsed.diagnostics);
        let diagnostic = diagnostics[0];
        assert_eq!(diagnostic.severity, cem_ml::diagnostics::Severity::Error);
        assert_eq!(
            diagnostic.details.as_ref().and_then(|details| {
                details.get("factKind").and_then(serde_json::Value::as_str)
            }),
            Some("declaration-duplicate")
        );
        assert_eq!(
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("name"))
                .and_then(serde_json::Value::as_str),
            Some("value")
        );
    }
}
