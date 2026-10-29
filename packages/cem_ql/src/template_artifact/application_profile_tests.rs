//! Build/runtime boundary proof, not a reduced WASM or a browser storefront.
use super::*;
use crate::{
    compile_profile::{measure, Stages},
    eval::{imported_cem_tree, retained_cem_node, AtomValue, ItemStream},
    render::{render_compiled_template, render_plan_to_html, TemplateData},
};
use std::{fs, path::PathBuf, sync::Arc};

const PROFILE: &str =
    include_str!("../../fixtures/component-application/shopping-cart/profile.json");
const VIEW: &str = include_str!("../../fixtures/component-application/shopping-cart/cart-view.cem");
const SOURCE_URI: &str = "app://shopping-cart/cart-view.cem";

fn fixture(name: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/component-application/shopping-cart")
            .join(name),
    )
    .unwrap()
}

fn options() -> CompileTemplateOptions {
    CompileTemplateOptions {
        host_bindings: vec!["response".into()],
        source_uri: Some(SOURCE_URI.into()),
        ..Default::default()
    }
}

fn assert_runtime_stages(stages: &Stages) {
    // The evaluator's function registry is a runtime dependency. Its assembly
    // is not source compilation and still needs profile-specific partitioning.
    let compiler: Vec<_> = stages
        .keys()
        .filter(|name| {
            name.starts_with("template/")
                || name.starts_with("template-artifact/")
                || name.starts_with("query/")
                || name.starts_with("type-check/")
        })
        .collect();
    assert!(
        compiler.is_empty(),
        "runtime invoked compiler stages: {compiler:?}"
    );
    assert!(
        stages.contains_key("render/body"),
        "a real render must execute"
    );
}

fn response(name: &str) -> TemplateData {
    // Parse the original API JSON natively. This never maps a serde_json::Value
    // into a replacement data/AST graph or passes application data through JS.
    let uri = format!("app://shopping-cart/responses/{name}");
    let tree = cem_ml::import::import_data_bytes(
        fixture(name).as_bytes(),
        "application/json",
        "cem",
        &uri,
    )
    .expect("valid JSON fixture");
    let input = imported_cem_tree(tree.clone());
    let owner = retained_cem_node(&input).unwrap();
    assert!(Arc::ptr_eq(owner.owner(), &tree));
    assert_eq!(tree.source_uri(), uri);
    TemplateData::default().with_binding("response", ItemStream::once(input))
}

fn render(artifact: &TemplateArtifact, name: &str) -> String {
    let plan = render_compiled_template(artifact, &response(name));
    assert!(
        plan.diagnostics.is_empty(),
        "{name}: {:?}",
        plan.diagnostics
    );
    render_plan_to_html(&plan)
}

#[test]
fn shopping_cart_reloads_once_and_renders_changing_native_json_without_compilation() {
    for mode in [
        TemplateArtifactSourceMapMode::Dev,
        TemplateArtifactSourceMapMode::Prod,
    ] {
        let (compiled, build) = measure(|| compile_template_artifact(VIEW, &options(), mode));
        // Positive control: instrumentation must observe the source compiler
        // during the build before zero runtime compiler calls count as proof.
        assert_eq!(build["template/tokenize"].calls, 1);
        assert!(build["query/parse"].calls > 0);
        let context = TemplateArtifactLoadContext {
            expected_source_hash: Some(compiled.identity.source_hash.clone()),
            host_bindings: options().host_bindings,
            source_map_mode: mode,
        };
        let bytes = compiled.bytes.clone();
        let pinned_content_hash = compiled.content_hash.clone();
        let (outputs, runtime) = measure(|| {
            // Admission needs only the build's pinned identities and binary IR.
            // Original CEM-ML input is not supplied to the reload/render path.
            let envelope = CompiledTemplateArtifact::from_bytes(bytes).unwrap();
            assert_eq!(
                envelope.content_hash, pinned_content_hash,
                "build-pinned artifact bytes"
            );
            let artifact = envelope.reload(&context).unwrap();
            assert!(
                artifact.diagnostics.is_empty(),
                "{:?}",
                artifact.diagnostics
            );
            [
                "cart.json",
                "updated.json",
                "removed.json",
                "empty.json",
                "partial-error.json",
                "failed.json",
            ]
            .map(|name| render(&artifact, name))
        });
        assert_runtime_stages(&runtime);
        eprintln!(
            "cart/{mode:?}: source_compilation=0, renders={}, general_registry_assemblies={}",
            runtime["render/body"].calls,
            runtime
                .get("stdlib/assemble-registry")
                .map_or(0, |stage| stage.calls),
        );
        let [cart, updated, removed, empty, partial, failed] = outputs;
        assert!(
            cart.contains("Tea") && cart.contains("value=\"2\"") && cart.contains("USD 18.00"),
            "{cart}"
        );
        assert!(
            updated.contains("value=\"3\"") && updated.contains("USD 32.00"),
            "{updated}"
        );
        assert!(cart.contains("Delivery estimate: 2 days"));
        assert!(!partial.contains("Delivery estimate:"));
        assert!(updated.contains("Coffee &lt;img"), "{updated}");
        assert!(!updated.contains("<img"));
        assert_eq!(updated.matches("<li ").count(), 2);
        assert!(!removed.contains("Tea"));
        assert!(removed.contains("Coffee") && removed.contains("USD 5.00"));
        assert_eq!(removed.matches("<li ").count(), 1);
        assert!(empty.contains("Your cart is empty") && empty.contains("USD 0.00"));
        assert!(!empty.contains("<li "));
        assert!(
            partial.contains("The shipping estimate is unavailable.")
                && partial.contains("Tea")
                && partial.contains("USD 9.00")
        );
        assert!(failed.contains("Please sign in again.") && failed.contains("Cart unavailable"));
        assert!(!failed.contains("aria-label=\"Total\"") && !failed.contains("<li "));
    }
}

fn component_tags(nodes: &[TemplateNode], tags: &mut BTreeSet<String>) {
    for node in nodes {
        match node {
            TemplateNode::Element { tag, children, .. } => {
                if tag.starts_with("cem-") {
                    tags.insert(tag.clone());
                }
                component_tags(children, tags);
            }
            TemplateNode::Result { children, .. }
            | TemplateNode::If { children, .. }
            | TemplateNode::ForEach { children, .. } => component_tags(children, tags),
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    component_tags(&branch.children, tags);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn shopping_cart_manifest_matches_compiled_view_and_shared_component_artifacts() {
    // JSON here is build-control metadata, never a replacement application AST.
    let profile: serde_json::Value = serde_json::from_str(PROFILE).unwrap();
    assert_eq!(profile["version"], "cem-application-reference/1");
    assert_eq!(profile["entryTemplate"], "cart-view.cem");
    assert_eq!(
        profile["hostBindings"],
        serde_json::json!(options().host_bindings)
    );
    assert_eq!(profile["runtimeFormats"], serde_json::json!(["json"]));
    assert_eq!(profile["deployment"]["dynamicTemplateSource"], false);
    for file in profile["responses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
    {
        assert!(!fixture(file).is_empty());
    }
    for key in ["schema", "operations"] {
        assert!(!fixture(profile["graphql"][key].as_str().unwrap()).is_empty());
    }
    let compiled = compile_template_artifact(VIEW, &options(), TemplateArtifactSourceMapMode::Dev);
    let artifact = compiled
        .reload(&TemplateArtifactLoadContext {
            expected_source_hash: Some(compiled.identity.source_hash.clone()),
            host_bindings: options().host_bindings,
            source_map_mode: TemplateArtifactSourceMapMode::Dev,
        })
        .unwrap();
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let expected: BTreeSet<_> = profile["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| tag.as_str().unwrap().to_owned())
        .collect();
    let mut actual = BTreeSet::new();
    component_tags(&artifact.nodes, &mut actual);
    assert_eq!(actual, expected, "reference manifest/component drift");
    for tag in actual {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../cem-components/src/components/{tag}/{tag}.xhtml"
        ));
        let declaration = fs::read_to_string(path).unwrap();
        let templates =
            cem_ml::legacy_custom_element::extract_html_template_fragments(&declaration);
        let template = templates
            .iter()
            .find(|template| template.attributes.get("id") == Some(&tag))
            .unwrap();
        assert_eq!(
            template.attributes.get("type").map(String::as_str),
            Some("text/cem-ml")
        );
        let compiled = compile_template_artifact(
            &template.body,
            &CompileTemplateOptions::default(),
            TemplateArtifactSourceMapMode::Prod,
        );
        let loaded = CompiledTemplateArtifact::from_bytes(compiled.bytes)
            .unwrap()
            .reload(&TemplateArtifactLoadContext {
                expected_source_hash: Some(compiled.identity.source_hash.clone()),
                host_bindings: vec![],
                source_map_mode: TemplateArtifactSourceMapMode::Prod,
            })
            .unwrap();
        assert!(
            loaded.diagnostics.is_empty(),
            "{tag}: {:?}",
            loaded.diagnostics
        );
        assert!(
            !loaded.stylesheets.is_empty(),
            "canonical {tag} retains native CSS"
        );
        for stylesheet in loaded.stylesheets {
            assert!(!stylesheet.css_tree().ast().nodes.is_empty());
        }
    }
}

#[test]
fn shopping_cart_pinned_build_identity_refuses_changed_bytes_bindings_and_versions() {
    let compiled = compile_template_artifact(VIEW, &options(), TemplateArtifactSourceMapMode::Prod);
    let context = TemplateArtifactLoadContext {
        expected_source_hash: Some(compiled.identity.source_hash.clone()),
        host_bindings: options().host_bindings,
        source_map_mode: TemplateArtifactSourceMapMode::Prod,
    };
    let mut tampered = compiled.clone();
    *tampered.bytes.last_mut().unwrap() ^= 1;
    assert_eq!(
        tampered.reload(&context).unwrap_err().code,
        "cem.ql.template_artifact_hash_mismatch"
    );
    let wrong_source = TemplateArtifactLoadContext {
        expected_source_hash: Some(ContentHash::from_blake3(b"another-build")),
        ..context.clone()
    };
    assert_eq!(
        compiled.reload(&wrong_source).unwrap_err().code,
        "cem.ql.template_artifact_hash_mismatch"
    );
    let wrong_bindings = TemplateArtifactLoadContext {
        host_bindings: vec!["other".into()],
        ..context.clone()
    };
    assert_eq!(
        compiled.reload(&wrong_bindings).unwrap_err().code,
        "cem.cc.policy_mismatch"
    );
    let (mut identity, payload) = deserialize_artifact_bytes(&compiled.bytes).unwrap();
    identity.cem_ql_version = "incompatible-build".into();
    let other = CompiledTemplateArtifact::from_bytes(serialize_artifact_bytes(&identity, &payload))
        .unwrap();
    assert_eq!(
        other.reload(&context).unwrap_err().code,
        "cem.ql.template_artifact_hash_mismatch"
    );
}

#[test]
fn shopping_cart_bad_json_is_refused_and_valid_input_retains_native_ownership() {
    for source in ["{", "[1,]", "{\"data\":"] {
        assert!(cem_ml::import::import_data_bytes(
            source.as_bytes(),
            "application/json",
            "cem",
            "app://shopping-cart/bad.json"
        )
        .is_err());
    }
    let tree = cem_ml::import::import_data_bytes(
        fixture("cart.json").as_bytes(),
        "application/json",
        "cem",
        "app://shopping-cart/cart.json",
    )
    .unwrap();
    let node = imported_cem_tree(tree);
    assert_eq!(
        node.view().unwrap().field("kind").unwrap()[0].atom(),
        Some(AtomValue::String("document".into()))
    );
}

fn pin(compiled: &CompiledTemplateArtifact) -> PinnedTemplateArtifactContext {
    PinnedTemplateArtifactContext {
        content_hash: compiled.content_hash.header_value(),
        source_hash: compiled.identity.source_hash.header_value(),
        cem_ml_version: cem_ml::VERSION.into(),
        cem_ql_version: crate::VERSION.into(),
        host_bindings: options().host_bindings,
        source_map_mode: compiled.identity.source_map_mode,
    }
}

#[test]
fn pinned_native_deployment_admits_only_the_independent_build_contract() {
    for mode in [
        TemplateArtifactSourceMapMode::Dev,
        TemplateArtifactSourceMapMode::Prod,
    ] {
        let compiled = compile_template_artifact(VIEW, &options(), mode);
        let context = pin(&compiled);
        let bytes = compiled.bytes.clone();
        let (html, stages) = measure(|| {
            let loaded = CompiledTemplateArtifact::from_bytes(bytes)
                .unwrap()
                .reload_pinned(&context)
                .unwrap();
            render(&loaded, "updated.json")
        });
        assert_runtime_stages(&stages);
        assert!(html.contains("USD 32.00"));
        for changed in ["", "unknown:abc", "cem-bin/1+blake3:00"] {
            for field in ["contentHash", "sourceHash", "cemMlVersion", "cemQlVersion"] {
                let mut json = serde_json::to_value(&context).unwrap();
                json[field] = changed.into();
                let bad: PinnedTemplateArtifactContext = serde_json::from_value(json).unwrap();
                assert_eq!(
                    compiled.reload_pinned(&bad).unwrap_err().code,
                    "cem.ql.template_artifact_hash_mismatch"
                );
            }
        }
        let mut bad = context.clone();
        bad.host_bindings = vec![];
        assert_eq!(
            compiled.reload_pinned(&bad).unwrap_err().code,
            "cem.cc.policy_mismatch"
        );
        bad = context.clone();
        bad.source_map_mode = if mode == TemplateArtifactSourceMapMode::Dev {
            TemplateArtifactSourceMapMode::Prod
        } else {
            TemplateArtifactSourceMapMode::Dev
        };
        assert_eq!(
            compiled.reload_pinned(&bad).unwrap_err().code,
            "cem.cc.policy_mismatch"
        );
        let mut json = serde_json::to_value(&context).unwrap();
        json["source"] = "must not become a source fallback".into();
        assert!(serde_json::from_value::<PinnedTemplateArtifactContext>(json).is_err());
    }
}

#[test]
fn pinned_deployment_refuses_a_matching_but_invalid_build() {
    let compiled = compile_template_artifact(
        "{p | {$missing}}",
        &options(),
        TemplateArtifactSourceMapMode::Prod,
    );
    assert_eq!(
        compiled.reload_pinned(&pin(&compiled)).unwrap_err().code,
        "cem.ql.template_artifact_invalid_build"
    );
}
