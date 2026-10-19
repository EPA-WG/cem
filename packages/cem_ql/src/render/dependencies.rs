//! Opt-in native compilation receipt. This entrypoint evaluates the ordinary
//! compiler once; it does not retrieve/store artifacts or suppress evaluation.
use super::{
    compile_template_module_closure, CompileTemplateOptions, TemplateArtifact,
    TemplateModuleClosure,
};
use cem_ml::{
    content_cache::{ContentHash, HASH_SCHEME},
    dependency_identity::{
        DependencyClosureState, DependencyIdentity, DependencyIdentityError, DependencyLayer,
        DependencySnapshot, DependencyVersion, ImportDependency, SourceDependency,
    },
};
use std::collections::{BTreeMap, BTreeSet};

pub struct TemplateDependencyCompilation {
    pub artifact: TemplateArtifact,
    pub identity: Result<DependencyIdentity, DependencyIdentityError>,
}

/// The resolver must attest a complete, current closure using a nonempty stable
/// policy/freshness stamp. Absent stamps are opaque, not a default global context.
/// Identity covers compilation only: CSS loading and render inputs have separate
/// identities, and every reuse still requires live owner/runtime admission.
pub fn compile_template_module_closure_with_dependencies(
    source: &str,
    closure: &TemplateModuleClosure,
    options: &CompileTemplateOptions,
) -> TemplateDependencyCompilation {
    let artifact = compile_template_module_closure(source, closure, options);
    let identity = if artifact
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        Err(DependencyIdentityError::IncompleteClosure)
    } else {
        compilation_identity(closure, options)
    };
    TemplateDependencyCompilation { artifact, identity }
}

pub(crate) fn compilation_identity(
    closure: &TemplateModuleClosure,
    options: &CompileTemplateOptions,
) -> Result<DependencyIdentity, DependencyIdentityError> {
    if closure.modules.len() > cem_ml::dependency_identity::MAX_DEPENDENCY_IMPORTS {
        return Err(DependencyIdentityError::Bounds);
    }
    let resource = |header: &str| {
        let hex = header
            .strip_prefix(HASH_SCHEME)
            .and_then(|s| s.strip_prefix(':'))
            .ok_or(DependencyIdentityError::InvalidHash)?;
        Ok(SourceDependency {
            content_type: "application/cem".into(),
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.into()),
            version: DependencyVersion::Content(ContentHash {
                scheme: HASH_SCHEME.into(),
                hex: hex.into(),
            }),
        })
    };
    let mut sources = BTreeMap::from([(
        closure.root_uri.clone(),
        resource(&closure.root_content_hash)?,
    )]);
    let mut imports = Vec::new();
    for module in &closure.modules {
        let source = resource(&module.content_hash)?;
        if sources
            .get(&module.uri)
            .is_some_and(|existing| existing != &source)
        {
            return Err(DependencyIdentityError::InvalidClosure);
        }
        sources.insert(module.uri.clone(), source);
        imports.push(ImportDependency {
            parent_uri: module
                .parent_uri
                .clone()
                .unwrap_or_else(|| closure.root_uri.clone()),
            specifier: module.alias.clone(),
            resolved_uri: module.uri.clone(),
        });
    }
    let bindings = options
        .host_bindings
        .iter()
        .filter(|name| !name.is_empty())
        .collect::<BTreeSet<_>>();
    let stamp =
        |value: &str| DependencyVersion::Content(ContentHash::from_blake3(value.as_bytes()));
    DependencySnapshot {
        layer: DependencyLayer::TemplateCompilation,
        engine_version: format!("cem-ml/{};cem-ql/{}", cem_ml::VERSION, crate::VERSION),
        format_version: crate::template_artifact::CEM_TEMPLATE_IR_FORMAT.into(),
        closure_state: DependencyClosureState::Ready,
        root_uri: closure.root_uri.clone(),
        sources,
        imports,
        options: BTreeMap::from([
            (
                "entrypoint".into(),
                if closure.entrypoint.is_empty() {
                    "body".into()
                } else {
                    closure.entrypoint.clone()
                },
            ),
            (
                "host-bindings".into(),
                serde_json::to_string(&bindings).expect("binding names serialize"),
            ),
            // Compilation currently appends imported declarations/stylesheets in
            // preflight order. Graph enumeration alone cannot erase that axis.
            (
                "module-order".into(),
                serde_json::to_string(
                    &closure
                        .modules
                        .iter()
                        .map(|m| (&m.parent_uri, &m.alias, &m.uri))
                        .collect::<Vec<_>>(),
                )
                .expect("module identities serialize"),
            ),
            (
                "skip-cemt-function-bodies".into(),
                options.skip_cemt_function_bodies.to_string(),
            ),
        ]),
        // The preflight resolver stamp already includes module-map/execution policy.
        policy: if closure.resolver_policy_stamp.is_empty() {
            DependencyVersion::Opaque
        } else {
            stamp(&closure.resolver_policy_stamp)
        },
        resolution: if closure.resolver_policy_stamp.is_empty() {
            DependencyVersion::Opaque
        } else {
            stamp(&closure.resolver_policy_stamp)
        },
        // Compilation has no consumer DOM scope; this explicit domain prevents a
        // compilation key from being mistaken for a scoped stylesheet/render key.
        scope: stamp("cem-template-compilation/no-consumer-dom-scope/1"),
        inputs: BTreeMap::new(),
    }
    .identity()
}
